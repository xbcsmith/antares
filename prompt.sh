#!/usr/bin/env bash

PHASE_NUMBER=$1
IMPLEMENTATION_PLAN=$2

PHASE_TEXT=$(grep "### Phase ${PHASE_NUMBER}" docs/explanation/character_recruitment_implementation_plan.md | awk -F "#" '{print $NF}')

echo "You are an elite Rust Game Developer. Implement ${PHASE_TEXT} from ${IMPLEMENTATION_PLAN} DO NOT SKIP TASKS or DELIVERABLES. THINK HARD and follow the rules in @PLAN.md"
