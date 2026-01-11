#!/usr/bin/env bash

PHASE_NUMBER=$1
IMPLEMENTATION_PLAN=$2
PLAN=$(basename ${IMPLEMENTATION_PLAN})
PHASE_TEXT=$(grep "### Phase ${PHASE_NUMBER}" ${IMPLEMENTATION_PLAN} | awk -F "#" '{print $NF}')

echo "You are an elite Rust Game Developer. Implement ${PHASE_TEXT} from @${PLAN} DO NOT SKIP TASKS or DELIVERABLES. THINK HARD and follow the rules in @PLAN.md"
