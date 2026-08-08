#!/usr/bin/env python3
"""Generate Rust code for criterion mapping from CSV."""
import csv

with open('/home/jamin/Documents/RGAA/grille-rgaa-106.csv', 'r') as f:
    reader = csv.DictReader(f)
    criteria = list(reader)

print("// Auto-generated from grille-rgaa-106.csv")
print("use crate::models::Classification;")
print()
print("pub fn get_criterion_info(criterion_id: &str) -> Option<(&'static str, Classification)> {")
print("    match criterion_id {")

for row in criteria:
    crit_id = row['critere_id']
    title = row['titre'].replace('"', '\\"')
    classification = row['classification_proposee'].split()[0]  # DETERMINISTE, IA_ASSISTE, MANUEL
    rust_class = classification.capitalize()  # Deterministe, IaAssiste, Manuel
    print(f'        "{crit_id}" => Some(("{title}", Classification::{rust_class})),')

print("        _ => None,")
print("    }")
print("}")