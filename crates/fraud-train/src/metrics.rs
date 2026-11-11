fn ranked(probs: &[f32], labels: &[bool]) -> Vec<(f32, usize, usize)> {
    let mut order: Vec<usize> = (0..probs.len()).collect();
    order.sort_unstable_by(|&a, &b| probs[b].total_cmp(&probs[a]));
    let mut groups: Vec<(f32, usize, usize)> = Vec::new();
    for i in order {
        match groups.last_mut() {
            Some(group) if group.0 == probs[i] => {}
            _ => groups.push((probs[i], 0, 0)),
        }
        let group = groups.last_mut().unwrap();
        if labels[i] { group.1 += 1 } else { group.2 += 1 }
    }
    groups
}

fn frauds(labels: &[bool]) -> usize {
    labels.iter().filter(|&&y| y).count()
}

pub fn auc_pr(probs: &[f32], labels: &[bool]) -> f64 {
    let positives = frauds(labels) as f64;
    let (mut tp, mut fp, mut area) = (0.0, 0.0, 0.0);
    for (_, frauds, legit) in ranked(probs, labels) {
        tp += frauds as f64;
        fp += legit as f64;
        area += frauds as f64 / positives * tp / (tp + fp);
    }
    area
}

pub fn auroc(probs: &[f32], labels: &[bool]) -> f64 {
    let positives = frauds(labels) as f64;
    let negatives = labels.len() as f64 - positives;
    let (mut frauds_above, mut area) = (0.0, 0.0);
    for (_, frauds, legit) in ranked(probs, labels) {
        area += legit as f64 * (frauds_above + frauds as f64 / 2.0);
        frauds_above += frauds as f64;
    }
    area / (positives * negatives)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Confusion {
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
}

impl Confusion {
    pub fn at(probs: &[f32], labels: &[bool], threshold: f32) -> Self {
        let mut c = Confusion { true_positives: 0, false_positives: 0, false_negatives: 0 };
        for (&p, &fraud) in probs.iter().zip(labels) {
            match (p >= threshold, fraud) {
                (true, true) => c.true_positives += 1,
                (true, false) => c.false_positives += 1,
                (false, true) => c.false_negatives += 1,
                (false, false) => {}
            }
        }
        c
    }

    pub fn precision(&self) -> f64 {
        let flagged = self.true_positives + self.false_positives;
        if flagged == 0 { 1.0 } else { self.true_positives as f64 / flagged as f64 }
    }

    pub fn recall(&self) -> f64 {
        let frauds = self.true_positives + self.false_negatives;
        if frauds == 0 { 0.0 } else { self.true_positives as f64 / frauds as f64 }
    }
}

/* Returns 2.0 (flag nothing) when no threshold beats it. */
pub fn cheapest_threshold(probs: &[f32], labels: &[bool], fp_cost: f64, fn_cost: f64) -> f32 {
    let total_frauds = frauds(labels);
    let (mut tp, mut fp) = (0, 0);
    let mut best = (fn_cost * total_frauds as f64, 2.0);
    for (p, frauds, legit) in ranked(probs, labels) {
        tp += frauds;
        fp += legit;
        let cost = fp_cost * fp as f64 + fn_cost * (total_frauds - tp) as f64;
        if cost < best.0 {
            best = (cost, p);
        }
    }
    best.1
}
