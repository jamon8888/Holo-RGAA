# CI Integration Runbook

This runbook shows how to integrate RGAA audits into your CI/CD pipeline.

## GitHub Actions

### Basic Workflow

```yaml
# .github/workflows/a11y.yml
name: Accessibility Audit

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  rgaa-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install rgaa-cli
        run: |
          curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash
          echo "$HOME/.local/bin" >> $GITHUB_PATH

      - name: Run RGAA audit
        run: |
          rgaa audit analyze --url ${{ vars.AUDIT_URL || 'https://example.test' }} \
            --format sarif --output results.sarif

      - name: Upload SARIF results
        uses: github/code-scanning-action@v3
        with:
          sarif_file: results.sarif
          category: rgaa-audit
```

### Full Workflow with Policy Gate

```yaml
# .github/workflows/rgaa.yml
name: RGAA Accessibility Audit

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  AUDIT_URL: https://staging.example.test
  MIN_COMPLIANCE: 85.0

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup rgaa-cli
        run: |
          curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash
          echo "$HOME/.local/bin" >> $GITHUB_PATH

      - name: Run RGAA audit
        id: audit
        run: |
          rgaa audit analyze \
            --url "${{ env.AUDIT_URL }}" \
            --format json \
            --output audit-results.json

      - name: Check policy compliance
        id: policy
        run: |
          rgaa audit policy --input audit-results.json
          EXIT_CODE=$?
          
          if [ $EXIT_CODE -eq 0 ]; then
            echo "status=compliant" >> $GITHUB_OUTPUT
          else
            echo "status=non_compliant" >> $GITHUB_OUTPUT
          fi

      - name: Generate HTML report
        if: always()
        run: |
          rgaa audit report \
            --input audit-results.json \
            --format html \
            --output audit-report.html

      - name: Upload report artifact
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: rgaa-report
          path: audit-report.html
          retention-days: 30

      - name: Update status check
        if: always()
        run: |
          echo "Audit completed with status: ${{ steps.policy.outputs.status }}"
```

## GitLab CI

```yaml
# .gitlab-ci.yml
stages:
  - accessibility

rgaa_audit:
  stage: accessibility
  image: curlimages/curl:latest
  before_script:
    - curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash
    - export PATH="$HOME/.local/bin:$PATH"
  script:
    - rgaa audit analyze --url "$AUDIT_URL" --format sarif --output results.sarif
  artifacts:
    reports:
      sarif: results.sarif
    paths:
      - results.sarif
    expire_in: 1 week
  variables:
    AUDIT_URL: "https://example.test"
```

## Jenkins

```groovy
// Jenkinsfile
pipeline {
    agent any
    
    environment {
        AUDIT_URL = 'https://example.test'
    }
    
    stages {
        stage('Accessibility Audit') {
            steps {
                sh '''
                    curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash
                    export PATH="$HOME/.local/bin:$PATH"
                    
                    rgaa audit analyze \
                        --url "${AUDIT_URL}" \
                        --format sarif \
                        --output results.sarif
                '''
            }
            post {
                always {
                    recordIssues(
                        tools: [scanForIssues(
                            tool: sarif(
                                pattern: 'results.sarif'
                            )
                        )]
                    )
                }
            }
        }
    }
}
```

## GitHub Actions with Delta Tracking

Track accessibility regressions over time:

```yaml
# .github/workflows/rgaa-compare.yml
name: RGAA Delta Check

on:
  pull_request:
    branches: [main]

jobs:
  compare:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install rgaa-cli
        run: |
          curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash
          echo "$HOME/.local/bin" >> $GITHUB_PATH

      - name: Get base audit
        run: |
          # Get the commit SHA of the base branch
          BASE_SHA=$(git merge-base HEAD main)
          git checkout $BASE_SHA
          
          rgaa audit analyze --url ${{ env.AUDIT_URL }} --format json -o base-audit.json
          git checkout -

      - name: Get head audit
        run: |
          rgaa audit analyze --url ${{ env.AUDIT_URL }} --format json -o head-audit.json

      - name: Compare audits
        run: |
          # Extract compliance scores
          BASE_RATE=$(cat base-audit.json | jq '.taux_global')
          HEAD_RATE=$(cat head-audit.json | jq '.taux_global')
          
          echo "Base compliance: $BASE_RATE%"
          echo "Head compliance: $HEAD_RATE%"
          
          # Fail if regressed by more than 5%
          REGRESSION=$(echo "$BASE_RATE - $HEAD_RATE" | bc)
          if (( $(echo "$REGRESSION > 5" | bc -l) )); then
            echo "ERROR: Compliance regressed by $REGRESSION%"
            exit 1
          fi
```

## CircleCI

```yaml
# .circleci/config.yml
version: 2.1

executors:
  rgaa-executor:
    docker:
      - image: cimg/base:stable
    working_directory: ~/project

jobs:
  rgaa-audit:
    executor: rgaa-executor
    steps:
      - checkout
      - run:
          name: Install rgaa-cli
          command: |
            curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash
            echo 'export PATH="$HOME/.local/bin:$PATH"' >> $BASH_ENV
      - run:
          name: Run RGAA audit
          command: |
            source $BASH_ENV
            rgaa audit analyze --url $AUDIT_URL --format sarif --output results.sarif
      - store_artifacts:
          path: results.sarif

workflows:
  version: 2
  audit:
    jobs:
      - rgaa-audit
```

## GitHub Code Scanning Alerts

To see RGAA results directly in GitHub's Security tab:

1. Ensure SARIF upload is configured (see basic workflow above)
2. Results appear under **Security > Code scanning alerts**
3. Filter by `RGAA` tool name

## Notifications

### Slack Integration

```yaml
- name: Notify Slack on failure
  if: failure()
  uses: slackapi/slack-github-action@v1
  with:
    channel-id: 'accessibility'
    payload: |
      {
        "text": "RGAA Audit Failed",
        "blocks": [
          {
            "type": "section",
            "text": {
              "type": "mrkdwn",
              "text": "*RGAA Audit Failed*\n<${{ github.server_url }}/${{ github.repository }}/actions/runs/${{ github.run_id }}|View Run>"
            }
          }
        ]
      }
```

## Best Practices

1. **Run on every PR** - Catch accessibility issues before they reach production
2. **Set realistic thresholds** - 100% compliance is rarely achievable; aim for 85%+
3. **Track trends** - Use SARIF historical data to spot regressions
4. **Include manual testing** - Automated tests cover ~70% of RGAA; manual review needed for rest
5. **Prioritize critical issues** - Focus on critical severity findings first
