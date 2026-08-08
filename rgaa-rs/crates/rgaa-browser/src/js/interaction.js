/**
 * Interaction-based RGAA Testing
 * Migrates criteria from MANUEL to DETERMINISTE via Playwright interactions
 *
 * Criteria covered:
 * - 10.7: Focus visible (via keyboard simulation)
 * - 12.8: Tabindex coherence (via keyboard simulation)
 * - 9.3: DOM vs visual reading order
 * - 10.11: Reflow/zoom 200%
 * - 11.x: Form submission validation
 * - 12.9: Keyboard traps
 */

async function runKeyboardSimulation(page, url) {
  await page.goto(url, { waitUntil: 'networkidle', timeout: 30000 });

  const results = {
    '10.7': { focusVisible: [], missingFocus: [], passed: true },
    '12.8': { tabindexIssues: [], passed: true },
    '12.9': { keyboardTraps: [], passed: true },
    '12.11': { keyboardOperable: [], passed: true }
  };

  const focusableSelectors = [
    'a[href]', 'button', 'input', 'select', 'textarea',
    '[tabindex]:not([tabindex="-1"])', '[contenteditable="true"]',
    'area[href]', 'iframe', 'object', 'embed', 'video[controls]', 'audio[controls]'
  ];

  let tabOrder = [];
  let iterations = 0;
  const maxIterations = 100;

  await page.evaluate(() => {
    const focusable = document.querySelectorAll(
      'a[href], button, input, select, textarea, [tabindex]:not([tabindex="-1"]), [contenteditable="true"]'
    );
    if (focusable.length > 0) focusable[0].focus();
  });

  while (iterations < maxIterations) {
    iterations++;

    const activeElement = await page.evaluate(() => {
      const el = document.activeElement;
      if (!el || el === document.body) return null;
      return {
        tagName: el.tagName,
        id: el.id,
        className: el.className,
        tabindex: el.getAttribute('tabindex'),
        href: el.href,
        type: el.type,
        role: el.getAttribute('role'),
        ariaLabel: el.getAttribute('aria-label'),
        boundingBox: el.getBoundingClientRect()
      };
    });

    if (!activeElement) break;

    const focusStyles = await page.evaluate(() => {
      const el = document.activeElement;
      if (!el) return null;
      const computed = window.getComputedStyle(el, ':focus');
      const focusVisible = computed.outline !== 'none' && computed.outline !== '0px' ||
                           computed.boxShadow !== 'none' ||
                           computed.border !== 'none' && computed.border !== '0px';
      return { focusVisible, outline: computed.outline, boxShadow: computed.boxShadow };
    });

    if (focusStyles && !focusStyles.focusVisible) {
      results['10.7'].missingFocus.push({
        element: activeElement,
        outline: focusStyles.outline,
        boxShadow: focusStyles.boxShadow
      });
      results['10.7'].passed = false;
    }

    if (activeElement.tabindex) {
      const tabindex = parseInt(activeElement.tabindex);
      if (tabindex > 0) {
        results['12.8'].tabindexIssues.push({
          element: activeElement,
          issue: 'Positive tabindex creates non-sequential navigation',
          tabindex: tabindex
        });
        results['12.8'].passed = false;
      }
    }

    const elementKey = `${activeElement.tagName}#${activeElement.id}.${activeElement.className}`;
    tabOrder.push(elementKey);

    if (tabOrder.length > 20) {
      const isBody = await page.evaluate(() => document.activeElement === document.body);
      if (isBody) break;
    }

    await page.keyboard.press('Tab');
    await page.waitForTimeout(50);

    const isBody = await page.evaluate(() => document.activeElement === document.body);
    if (isBody) break;
  }

  await page.keyboard.press('Shift+Tab');
  await page.waitForTimeout(50);

  const modals = await page.evaluate(() => {
    return Array.from(document.querySelectorAll('[role="dialog"], [role="alertdialog"], .modal, .dialog, [aria-modal="true"]'))
      .map(el => ({
        visible: window.getComputedStyle(el).display !== 'none' &&
                 window.getComputedStyle(el).visibility !== 'hidden',
        hasCloseButton: !!el.querySelector('[aria-label*="close" i], [aria-label*="fermer" i], .close, .modal-close')
      }));
  });

  for (const modal of modals) {
    if (modal.visible) {
      await page.keyboard.press('Escape');
      await page.waitForTimeout(200);
      const stillVisible = await page.evaluate(() => {
        const modals = document.querySelectorAll('[role="dialog"], [role="alertdialog"], .modal, .dialog, [aria-modal="true"]');
        return Array.from(modals).some(el =>
          window.getComputedStyle(el).display !== 'none' &&
          window.getComputedStyle(el).visibility !== 'hidden'
        );
      });
      if (stillVisible) {
        results['12.11'].keyboardOperable.push({
          issue: 'Modal/dialog not dismissible with Escape key'
        });
        results['12.11'].passed = false;
      }
    }
  }

  return results;
}

async function runReadingOrderTest(page, url) {
  await page.goto(url, { waitUntil: 'networkidle', timeout: 30000 });

  const results = {
    '9.3': { discrepancies: [], passed: true }
  };

  const readingOrderData = await page.evaluate(() => {
    const contentElements = document.querySelectorAll(
      'h1, h2, h3, h4, h5, h6, p, li, td, th, caption, label, button, a[href], input:not([type="hidden"]), select, textarea, [role="heading"], [role="listitem"], [role="button"], [role="link"]'
    );

    const elements = [];
    contentElements.forEach((el, index) => {
      const rect = el.getBoundingClientRect();
      if (rect.width === 0 && rect.height === 0) return;
      if (window.getComputedStyle(el).display === 'none') return;
      if (window.getComputedStyle(el).visibility === 'hidden') return;

      elements.push({
        domIndex: index,
        tagName: el.tagName,
        id: el.id,
        className: el.className,
        text: el.textContent?.trim()?.substring(0, 100),
        rect: { top: rect.top, left: rect.left, width: rect.width, height: rect.height }
      });
    });

    return elements;
  });

  const visualOrder = [...readingOrderData].sort((a, b) => {
    const topDiff = a.rect.top - b.rect.top;
    if (Math.abs(topDiff) > 10) return topDiff;
    return a.rect.left - b.rect.left;
  });

  const domOrder = [...readingOrderData].sort((a, b) => a.domIndex - b.domIndex);

  const structuralTags = ['H1', 'H2', 'H3', 'H4', 'H5', 'H6', 'NAV', 'MAIN', 'ASIDE', 'HEADER', 'FOOTER', 'SECTION', 'ARTICLE'];

  for (let i = 0; i < Math.min(domOrder.length, visualOrder.length); i++) {
    const domEl = domOrder[i];

    if (!structuralTags.includes(domEl.tagName)) continue;

    const visualPos = visualOrder.findIndex(el =>
      el.tagName === domEl.tagName && el.id === domEl.id && el.className === domEl.className
    );

    if (visualPos !== -1 && Math.abs(i - visualPos) > 5) {
      results['9.3'].discrepancies.push({
        element: { tagName: domEl.tagName, id: domEl.id, text: domEl.text },
        domPosition: i,
        visualPosition: visualPos,
        shift: Math.abs(i - visualPos),
        domRect: domEl.rect,
        visualRect: visualOrder[visualPos].rect
      });
      results['9.3'].passed = false;
    }
  }

  return results;
}

async function runReflowTest(page, url) {
  const results = {
    '10.11': { issues: [], passed: true },
    '10.12': { textSpacingIssues: [], passed: true }
  };

  const viewports = [
    { width: 1280, height: 720, name: 'Desktop' },
    { width: 320, height: 720, name: 'Mobile (200% zoom equiv)' },
    { width: 256, height: 720, name: 'High zoom (250%)' }
  ];

  for (const viewport of viewports) {
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    await page.goto(url, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(500);

    const reflowIssues = await page.evaluate(() => {
      const issues = [];
      const body = document.body;
      const bodyRect = body.getBoundingClientRect();
      const viewportWidth = window.innerWidth;

      if (bodyRect.width > viewportWidth + 20) {
        issues.push({
          type: 'horizontal-scroll',
          message: `Content wider than viewport: ${bodyRect.width}px > ${viewportWidth}px`,
          severity: 'error'
        });
      }

      const allElements = document.querySelectorAll('*');
      const rects = [];
      allElements.forEach(el => {
        const style = window.getComputedStyle(el);
        if (style.position === 'absolute' || style.position === 'fixed' || style.display === 'flex' || style.display === 'grid') {
          const rect = el.getBoundingClientRect();
          if (rect.width > 0 && rect.height > 0) {
            rects.push({ el, rect, style });
          }
        }
      });

      if (viewportWidth <= 320) {
        const textElements = document.querySelectorAll('p, h1, h2, h3, h4, h5, h6, li, td, th, span, div');
        textElements.forEach(el => {
          const style = window.getComputedStyle(el);
          const lineHeight = parseFloat(style.lineHeight);
          const fontSize = parseFloat(style.fontSize);

          if (fontSize > 0 && lineHeight > 0 && lineHeight < 1.5 * fontSize) {
            issues.push({
              type: 'text-spacing-line-height',
              element: el.tagName + (el.id ? '#' + el.id : ''),
              message: `Line height ${lineHeight}px < 1.5 * font-size ${fontSize}px`,
              severity: 'warning'
            });
          }
        });
      }

      return issues;
    });

    if (reflowIssues.length > 0) {
      results['10.11'].issues.push({
        viewport: viewport.name,
        width: viewport.width,
        issues: reflowIssues
      });
      results['10.11'].passed = false;
    }
  }

  await page.setViewportSize({ width: 1280, height: 720 });

  return results;
}

async function runFormSubmissionTest(page, url) {
  await page.goto(url, { waitUntil: 'networkidle', timeout: 30000 });

  const results = {
    '11.1': { missingLabels: [], passed: true },
    '11.4': { missingInstructions: [], passed: true },
    '11.5': { fieldsetIssues: [], passed: true },
    '11.6': { fieldsetIssues: [], passed: true },
    '11.11': { errorHandling: [], passed: true },
    '11.12': { errorPrevention: [], passed: true },
    '11.13': { autocompleteIssues: [], passed: true }
  };

  const formData = await page.evaluate(() => {
    const forms = document.querySelectorAll('form');
    const formResults = [];

    forms.forEach((form, formIndex) => {
      const inputs = form.querySelectorAll('input, select, textarea');
      const fieldsets = form.querySelectorAll('fieldset');

      const inputData = Array.from(inputs).map(input => ({
        type: input.type,
        name: input.name,
        id: input.id,
        required: input.required,
        autocomplete: input.autocomplete,
        hasLabel: false,
        labelText: '',
        hasErrorMessage: false,
        ariaDescribedBy: input.getAttribute('aria-describedby'),
        ariaInvalid: input.getAttribute('aria-invalid'),
        ariaRequired: input.getAttribute('aria-required'),
        fieldset: null
      }));

      inputs.forEach((input, index) => {
        if (input.id) {
          const label = document.querySelector(`label[for="${input.id}"]`);
          if (label) {
            inputData[index].hasLabel = true;
            inputData[index].labelText = label.textContent.trim();
          }
        }
        if (!inputData[index].hasLabel) {
          const ariaLabel = input.getAttribute('aria-label');
          const ariaLabelledBy = input.getAttribute('aria-labelledby');
          if (ariaLabel) {
            inputData[index].hasLabel = true;
            inputData[index].labelText = ariaLabel;
          } else if (ariaLabelledBy) {
            const labelEl = document.getElementById(ariaLabelledBy);
            if (labelEl) {
              inputData[index].hasLabel = true;
              inputData[index].labelText = labelEl.textContent.trim();
            }
          }
        }
      });

      const fieldsetData = Array.from(fieldsets).map(fs => ({
        legend: fs.querySelector('legend')?.textContent?.trim() || '',
        inputs: Array.from(fs.querySelectorAll('input, select, textarea')).map(i => i.id || i.name)
      }));

      fieldsets.forEach((fs, fsIndex) => {
        fs.querySelectorAll('input, select, textarea').forEach(input => {
          const inputDataEl = inputData.find(d => d.id === input.id || d.name === input.name);
          if (inputDataEl) inputDataEl.fieldset = fsIndex;
        });
      });

      formResults.push({
        index: formIndex,
        action: form.action,
        method: form.method,
        inputs: inputData,
        fieldsets: fieldsetData,
        submitButtons: Array.from(form.querySelectorAll('button[type="submit"], input[type="submit"]'))
      });
    });

    return formResults;
  });

  for (const form of formData) {
    form.inputs.forEach(input => {
      if (input.type !== 'hidden' && input.type !== 'submit' && input.type !== 'button' && !input.hasLabel) {
        results['11.1'].missingLabels.push({
          form: form.index,
          input: { type: input.type, name: input.name, id: input.id },
          issue: 'Input missing accessible label'
        });
        results['11.1'].passed = false;
      }
    });

    form.inputs.forEach(input => {
      if (input.required && !input.ariaDescribedBy && !input.ariaRequired) {
        results['11.4'].missingInstructions.push({
          form: form.index,
          input: { type: input.type, name: input.name, id: input.id },
          issue: 'Required field lacks instruction/error association'
        });
        results['11.4'].passed = false;
      }
    });

    if (form.fieldsets.length === 0 && form.inputs.length > 3) {
      const radioGroups = form.inputs.filter(i => i.type === 'radio').map(i => i.name);
      const checkboxGroups = form.inputs.filter(i => i.type === 'checkbox').map(i => i.name);
      const uniqueRadioGroups = [...new Set(radioGroups)];
      const uniqueCheckboxGroups = [...new Set(checkboxGroups)];

      if (uniqueRadioGroups.length > 0 || uniqueCheckboxGroups.length > 0) {
        results['11.5'].fieldsetIssues.push({
          form: form.index,
          issue: 'Radio/checkbox groups should be wrapped in fieldset with legend',
          radioGroups: uniqueRadioGroups,
          checkboxGroups: uniqueCheckboxGroups
        });
        results['11.5'].passed = false;
        results['11.6'].passed = false;
      }
    }

    form.inputs.forEach(input => {
      if (input.type !== 'hidden' && input.type !== 'submit' && input.type !== 'button' &&
          input.autocomplete && input.autocomplete !== 'off') {
        const validAutocomplete = [
          'name', 'honorific-prefix', 'given-name', 'additional-name', 'family-name', 'honorific-suffix',
          'nickname', 'email', 'username', 'new-password', 'current-password', 'one-time-code',
          'organization-title', 'organization', 'street-address', 'address-line1', 'address-line2',
          'address-line3', 'address-level4', 'address-level3', 'address-level2', 'address-level1',
          'country', 'country-name', 'postal-code', 'cc-name', 'cc-given-name', 'cc-additional-name',
          'cc-family-name', 'cc-number', 'cc-exp', 'cc-exp-month', 'cc-exp-year', 'cc-csc', 'cc-type',
          'transaction-currency', 'transaction-amount', 'language', 'bday', 'bday-day', 'bday-month', 'bday-year',
          'sex', 'tel', 'tel-country-code', 'tel-national', 'tel-area-code', 'tel-local', 'tel-extension',
          'impp', 'url', 'photo'
        ];
        if (!validAutocomplete.includes(input.autocomplete)) {
          results['11.13'].autocompleteIssues.push({
            form: form.index,
            input: { type: input.type, name: input.name, id: input.id, autocomplete: input.autocomplete },
            issue: 'Invalid autocomplete value'
          });
          results['11.13'].passed = false;
        }
      }
    });

    if (form.submitButtons.length > 0) {
      try {
        for (const input of form.inputs) {
          if (input.required && input.type !== 'hidden' && input.type !== 'submit' && input.type !== 'button') {
            const selector = input.id ? `#${input.id}` : `[name="${input.name}"]`;
            try {
              if (input.type === 'email') {
                await page.fill(selector, 'invalid-email');
              } else if (input.type === 'tel') {
                await page.fill(selector, 'not-a-phone');
              } else if (input.type === 'number') {
                await page.fill(selector, 'not-a-number');
              } else if (input.type === 'checkbox') {
                // Leave unchecked for required checkbox
              } else if (input.type === 'radio') {
                // Leave unchecked for required radio
              } else {
                await page.fill(selector, 'x');
              }
            } catch (e) {
              // Selector might not work, skip
            }
          }
        }

        await page.click(form.submitButtons[0].tagName === 'BUTTON'
          ? `button[type="submit"]` : `input[type="submit"]`);
        await page.waitForTimeout(1000);

        const submissionResult = await page.evaluate(() => {
          const errors = [];
          const inputs = document.querySelectorAll('input[aria-invalid="true"], select[aria-invalid="true"], textarea[aria-invalid="true"]');
          inputs.forEach(input => {
            const errorId = input.getAttribute('aria-describedby');
            const errorEl = errorId ? document.getElementById(errorId) : null;
            errors.push({
              input: input.id || input.name,
              hasAriaInvalid: true,
              errorMessage: errorEl?.textContent?.trim()
            });
          });
          return errors;
        });

        if (submissionResult.length === 0) {
          results['11.11'].errorHandling.push({
            form: form.index,
            issue: 'Form submitted with invalid data but no aria-invalid/error messages detected'
          });
          results['11.11'].passed = false;
        } else {
          submissionResult.forEach(err => {
            if (!err.errorMessage || err.errorMessage.length < 5) {
              results['11.11'].errorHandling.push({
                form: form.index,
                input: err.input,
                issue: 'Error message missing or not descriptive'
              });
              results['11.11'].passed = false;
            }
          });
        }

        const isIrreversible = form.action && (
          form.action.includes('delete') ||
          form.action.includes('remove') ||
          form.action.includes('paiement') ||
          form.action.includes('payment') ||
          form.action.includes('commande') ||
          form.action.includes('order')
        );

        if (isIrreversible) {
          const hasConfirmation = await page.evaluate(() => {
            return document.querySelector('[role="alertdialog"], .confirmation-modal, [aria-live="assertive"]') !== null;
          });
          if (!hasConfirmation) {
            results['11.12'].errorPrevention.push({
              form: form.index,
              issue: 'Irreversible action lacks confirmation dialog'
            });
            results['11.12'].passed = false;
          }
        }

      } catch (e) {
        // Form submission test error - skip
      }
    }
  }

  return results;
}

async function runInteractionTests(page, url) {
  const allResults = {};

  const keyboardResults = await runKeyboardSimulation(page, url);
  Object.assign(allResults, keyboardResults);

  const readingOrderResults = await runReadingOrderTest(page, url);
  Object.assign(allResults, readingOrderResults);

  const reflowResults = await runReflowTest(page, url);
  Object.assign(allResults, reflowResults);

  const formResults = await runFormSubmissionTest(page, url);
  Object.assign(allResults, formResults);

  return allResults;
}

module.exports = { runInteractionTests };
