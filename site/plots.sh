#!/usr/bin/env bash
# Régénère les figures du site avec le programme lui-même.
# Usage : SQN=/chemin/vers/sinequanon site/plots.sh
set -euo pipefail

SQN="${SQN:-sinequanon}"
OUT="$(dirname "$0")/plots"
mkdir -p "$OUT"

"$SQN" export -f "sin(x)" -f "x^2/4-2" --aire 0:0:3.14159 --tangente 1:2 --point 1:-2 \
  --svg "$OUT/annotations.svg"
"$SQN" export -f "x^3/6-x" -f "2cos(x)" --svg "$OUT/fonctions.svg"
"$SQN" export -c "2cos(3t);2sin(2t)" --t-min -3.1416 --t-max 3.1416 --svg "$OUT/parametree.svg"
"$SQN" export -p "2cos(3theta)" --t-min 0 --t-max 3.1416 --unite-mm 22 --svg "$OUT/polaire.svg"
"$SQN" export -f "log(x)" --log-x --origine-x 10 --svg "$OUT/log.svg"
"$SQN" export -f "tan(x)" -f "3sin(2x)/2" --svg "$OUT/syntaxe.svg"
"$SQN" export -p "2+cos(theta)" --png "$OUT/limacon.png" --px-par-mm 6
