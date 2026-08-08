#!/usr/bin/env node
/**
 * Phase 4: Widget ARIA Pattern Detection + WAI-ARIA APG Test Sequences
 * Detects and tests common ARIA widgets for RGAA compliance:
 * - Accordion (role="button" + aria-expanded)
 * - Tablist (role="tablist" / role="tab" / role="tabpanel")
 * - Combobox (role="combobox" + listbox)
 * - Menu (role="menu" / role="menuitem")
 * - Tree (role="tree" / role="treeitem")
 * 
 * Tests criteria:
 * - 7.1: Scripts compatible with AT (role, aria-*)
 * - 7.3: Keyboard control
 * - 12.11: Additional content via keyboard
 */

const { chromium } = require('playwright');

// ═══════════════════════════════════════════════════════════════
// WIDGET PATTERNS - Detection definitions
// ═══════════════════════════════════════════════════════════════

const WIDGET_PATTERNS = {
  accordion: {
    name: 'Accordéon',
    detect: `
      (() => {
        const results = [];
        // Pattern 1: button + aria-controls + aria-expanded
        document.querySelectorAll('button[aria-controls][aria-expanded]').forEach(el => {
          const panel = document.getElementById(el.getAttribute('aria-controls'));
          if (panel) {
            results.push({
              trigger: el.outerHTML.slice(0, 200),
              panel: panel.outerHTML.slice(0, 200),
              expanded: el.getAttribute('aria-expanded'),
              panelRole: panel.getAttribute('role'),
              panelVisible: window.getComputedStyle(panel).display !== 'none'
            });
          }
        });
        // Pattern 2: role="button" + aria-expanded
        document.querySelectorAll('[role="button"][aria-controls][aria-expanded]').forEach(el => {
          const panel = document.getElementById(el.getAttribute('aria-controls'));
          if (panel && !results.find(r => r.trigger === el.outerHTML.slice(0, 200))) {
            results.push({
              trigger: el.outerHTML.slice(0, 200),
              panel: panel.outerHTML.slice(0, 200),
              expanded: el.getAttribute('aria-expanded'),
              panelRole: panel.getAttribute('role'),
              panelVisible: window.getComputedStyle(panel).display !== 'none'
            });
          }
        });
        return results;
      })()
    `,
    ariaTests: (widget) => {
      const issues = [];
      if (!widget.trigger.includes('aria-expanded')) issues.push('Missing aria-expanded on trigger');
      if (!widget.trigger.includes('aria-controls')) issues.push('Missing aria-controls on trigger');
      if (!widget.panelRole) issues.push('Panel missing role (should be region or group)');
      return issues;
    },
    keyboardTests: async (page, widgetSelector) => {
      const issues = [];
      // Find the first trigger
      const firstTrigger = await page.$('button[aria-controls][aria-expanded], [role="button"][aria-controls][aria-expanded]');
      if (!firstTrigger) return { issues: ['No accordion trigger found for keyboard test'], tested: false };
      
      await firstTrigger.focus();
      
      // Test: Enter or Space should toggle
      await page.keyboard.press('Enter');
      await page.waitForTimeout(200);
      let expanded = await page.evaluate(el => el.getAttribute('aria-expanded'), firstTrigger);
      if (expanded !== 'true') issues.push('Enter key did not expand accordion');
      
      await page.keyboard.press('Enter');
      await page.waitForTimeout(200);
      expanded = await page.evaluate(el => el.getAttribute('aria-expanded'), firstTrigger);
      if (expanded !== 'false') issues.push('Enter key did not collapse accordion');
      
      // Test: ArrowDown should move to next trigger
      await page.keyboard.press('Enter'); // expand first
      await page.waitForTimeout(200);
      await page.keyboard.press('ArrowDown');
      await page.waitForTimeout(200);
      const focusedTag = await page.evaluate(() => {
        const el = document.activeElement;
        return { tag: el.tagName, role: el.getAttribute('role'), text: el.textContent?.slice(0, 50) };
      });
      if (focusedTag.tag !== 'BUTTON' && focusedTag.role !== 'button') {
        issues.push('ArrowDown did not move to next accordion trigger');
      }
      
      // Test: ArrowUp should move to previous trigger
      await page.keyboard.press('ArrowUp');
      await page.waitForTimeout(200);
      const focusedAfterUp = await page.evaluate(() => document.activeElement);
      const upTarget = await page.evaluate(el => el === firstTrigger, focusedAfterUp);
      if (!upTarget) issues.push('ArrowUp did not move to previous accordion trigger');
      
      // Test: Home should move to first trigger
      await page.keyboard.press('Home');
      await page.waitForTimeout(200);
      const homeTarget = await page.evaluate(el => el === firstTrigger, await page.evaluate(() => document.activeElement));
      if (!homeTarget) issues.push('Home key did not move to first accordion trigger');
      
      return { issues, tested: true };
    }
  },

  tablist: {
    name: 'Tablist',
    detect: `
      (() => {
        const results = [];
        document.querySelectorAll('[role="tablist"]').forEach(tablist => {
          const tabs = Array.from(tablist.querySelectorAll('[role="tab"]'));
          const panels = tabs.map(tab => {
            const panelId = tab.getAttribute('aria-controls');
            return panelId ? document.getElementById(panelId) : null;
          });
          if (tabs.length > 0) {
            results.push({
              tablistHtml: tablist.outerHTML.slice(0, 300),
              tabCount: tabs.length,
              tabs: tabs.map((t, i) => ({
                html: t.outerHTML.slice(0, 200),
                selected: t.getAttribute('aria-selected'),
                controls: t.getAttribute('aria-controls'),
                panelExists: !!panels[i],
                panelHidden: panels[i] ? panels[i].getAttribute('aria-hidden') : null,
                panelDisplay: panels[i] ? window.getComputedStyle(panels[i]).display : null
              }))
            });
          }
        });
        return results;
      })()
    `,
    ariaTests: (widget) => {
      const issues = [];
      for (const tab of widget.tabs) {
        if (!tab.html.includes('role="tab"')) issues.push(`Tab missing role="tab"`);
        if (!tab.controls) issues.push(`Tab missing aria-controls`);
        if (!tab.panelExists) issues.push(`Tab panel ${tab.controls} not found`);
        if (tab.selected === null) issues.push(`Tab missing aria-selected`);
      }
      return issues;
    },
    keyboardTests: async (page) => {
      const issues = [];
      const firstTab = await page.$('[role="tab"]');
      if (!firstTab) return { issues: ['No tab found for keyboard test'], tested: false };
      
      await firstTab.focus();
      
      // Test: ArrowRight should move to next tab
      await page.keyboard.press('ArrowRight');
      await page.waitForTimeout(200);
      const afterRight = await page.evaluate(() => {
        const el = document.activeElement;
        return { role: el.getAttribute('role'), index: el.getAttribute('aria-selected') };
      });
      if (afterRight.role !== 'tab') issues.push('ArrowRight did not move to next tab');
      
      // Test: ArrowLeft should move to previous tab
      await page.keyboard.press('ArrowLeft');
      await page.waitForTimeout(200);
      const afterLeft = await page.evaluate(() => document.activeElement.getAttribute('role'));
      if (afterLeft !== 'tab') issues.push('ArrowLeft did not move to previous tab');
      
      // Test: Home should move to first tab
      await page.keyboard.press('Home');
      await page.waitForTimeout(200);
      const homeEl = await page.evaluate(() => document.activeElement.getAttribute('role'));
      if (homeEl !== 'tab') issues.push('Home key did not move to first tab');
      
      // Test: End should move to last tab
      await page.keyboard.press('End');
      await page.waitForTimeout(200);
      const endEl = await page.evaluate(() => document.activeElement.getAttribute('role'));
      if (endEl !== 'tab') issues.push('End key did not move to last tab');
      
      // Test: Enter/Space should activate tab
      await page.keyboard.press('Home');
      await page.waitForTimeout(200);
      await page.keyboard.press('Enter');
      await page.waitForTimeout(200);
      const activated = await page.evaluate(() => {
        const tab = document.activeElement;
        return tab.getAttribute('aria-selected');
      });
      if (activated !== 'true') issues.push('Enter key did not activate tab');
      
      return { issues, tested: true };
    }
  },

  combobox: {
    name: 'Combobox',
    detect: `
      (() => {
        const results = [];
        document.querySelectorAll('[role="combobox"]').forEach(cb => {
          const listboxId = cb.getAttribute('aria-controls') || cb.getAttribute('aria-owns');
          const listbox = listboxId ? document.getElementById(listboxId) : null;
          results.push({
            html: cb.outerHTML.slice(0, 300),
            expanded: cb.getAttribute('aria-expanded'),
            autocomplete: cb.getAttribute('aria-autocomplete'),
            activedescendant: cb.getAttribute('aria-activedescendant'),
            listboxExists: !!listbox,
            listboxRole: listbox ? listbox.getAttribute('role') : null,
            optionCount: listbox ? listbox.querySelectorAll('[role="option"]').length : 0
          });
        });
        return results;
      })()
    `,
    ariaTests: (widget) => {
      const issues = [];
      if (!widget.html.includes('aria-expanded')) issues.push('Combobox missing aria-expanded');
      if (!widget.listboxExists) issues.push('Listbox not found');
      if (widget.listboxRole !== 'listbox') issues.push('Listbox missing role="listbox"');
      if (widget.optionCount === 0) issues.push('No options found in listbox');
      return issues;
    },
    keyboardTests: async (page) => {
      const issues = [];
      const combobox = await page.$('[role="combobox"]');
      if (!combobox) return { issues: ['No combobox found for keyboard test'], tested: false };
      
      await combobox.focus();
      
      // Test: ArrowDown should open listbox
      await page.keyboard.press('ArrowDown');
      await page.waitForTimeout(300);
      const afterDown = await page.evaluate(() => {
        const cb = document.querySelector('[role="combobox"]');
        const listboxId = cb.getAttribute('aria-controls') || cb.getAttribute('aria-owns');
        const listbox = listboxId ? document.getElementById(listboxId) : null;
        return {
          expanded: cb.getAttribute('aria-expanded'),
          listboxVisible: listbox ? window.getComputedStyle(listbox).display !== 'none' : false,
          focused: document.activeElement.getAttribute('role')
        };
      });
      if (afterDown.expanded !== 'true' && afterDown.focused !== 'option') {
        issues.push('ArrowDown did not open listbox or move to option');
      }
      
      // Test: Escape should close listbox
      await page.keyboard.press('Escape');
      await page.waitForTimeout(200);
      const afterEsc = await page.evaluate(() => {
        const cb = document.querySelector('[role="combobox"]');
        return cb.getAttribute('aria-expanded');
      });
      if (afterEsc === 'true') issues.push('Escape did not close listbox');
      
      return { issues, tested: true };
    }
  },

  menu: {
    name: 'Menu',
    detect: `
      (() => {
        const results = [];
        document.querySelectorAll('[role="menu"]').forEach(menu => {
          const items = Array.from(menu.querySelectorAll('[role="menuitem"], [role="menuitemcheckbox"], [role="menuitemradio"]'));
          if (items.length > 0) {
            results.push({
              menuHtml: menu.outerHTML.slice(0, 300),
              itemCount: items.length,
              items: items.map(item => ({
                html: item.outerHTML.slice(0, 200),
                role: item.getAttribute('role'),
                disabled: item.getAttribute('aria-disabled'),
                checked: item.getAttribute('aria-checked')
              }))
            });
          }
        });
        return results;
      })()
    `,
    ariaTests: (widget) => {
      const issues = [];
      for (const item of widget.items) {
        if (!item.role.includes('menuitem')) issues.push(`Menu item missing proper role, got: ${item.role}`);
      }
      return issues;
    },
    keyboardTests: async (page) => {
      const issues = [];
      const menuitem = await page.$('[role="menuitem"], [role="menuitemcheckbox"], [role="menuitemradio"]');
      if (!menuitem) return { issues: ['No menuitem found for keyboard test'], tested: false };
      
      await menuitem.focus();
      
      // Test: ArrowDown should move to next item
      await page.keyboard.press('ArrowDown');
      await page.waitForTimeout(200);
      const afterDown = await page.evaluate(() => document.activeElement.getAttribute('role'));
      if (!afterDown?.includes('menuitem')) issues.push('ArrowDown did not move to next menuitem');
      
      // Test: ArrowUp should move to previous item
      await page.keyboard.press('ArrowUp');
      await page.waitForTimeout(200);
      const afterUp = await page.evaluate(() => document.activeElement.getAttribute('role'));
      if (!afterUp?.includes('menuitem')) issues.push('ArrowUp did not move to previous menuitem');
      
      // Test: Home should move to first item
      await page.keyboard.press('Home');
      await page.waitForTimeout(200);
      const homeRole = await page.evaluate(() => document.activeElement.getAttribute('role'));
      if (!homeRole?.includes('menuitem')) issues.push('Home key did not move to first menuitem');
      
      // Test: End should move to last item
      await page.keyboard.press('End');
      await page.waitForTimeout(200);
      const endRole = await page.evaluate(() => document.activeElement.getAttribute('role'));
      if (!endRole?.includes('menuitem')) issues.push('End key did not move to last menuitem');
      
      // Test: Escape should close menu
      await page.keyboard.press('Escape');
      await page.waitForTimeout(200);
      const afterEsc = await page.evaluate(() => document.activeElement.getAttribute('role'));
      if (afterEsc?.includes('menuitem')) issues.push('Escape did not close menu');
      
      return { issues, tested: true };
    }
  },

  tree: {
    name: 'Tree',
    detect: `
      (() => {
        const results = [];
        document.querySelectorAll('[role="tree"]').forEach(tree => {
          const items = Array.from(tree.querySelectorAll('[role="treeitem"]'));
          if (items.length > 0) {
            results.push({
              treeHtml: tree.outerHTML.slice(0, 300),
              itemCount: items.length,
              items: items.map(item => ({
                html: item.outerHTML.slice(0, 200),
                expanded: item.getAttribute('aria-expanded'),
                selected: item.getAttribute('aria-selected'),
                level: item.getAttribute('aria-level')
              }))
            });
          }
        });
        return results;
      })()
    `,
    ariaTests: (widget) => {
      const issues = [];
      for (const item of widget.items) {
        if (!item.html.includes('role="treeitem"')) issues.push('Tree item missing role="treeitem"');
        if (item.level === null) issues.push('Tree item missing aria-level');
      }
      return issues;
    },
    keyboardTests: async (page) => {
      const issues = [];
      const treeitem = await page.$('[role="treeitem"]');
      if (!treeitem) return { issues: ['No treeitem found for keyboard test'], tested: false };
      
      await treeitem.focus();
      
      // Test: ArrowDown should move to next visible item
      await page.keyboard.press('ArrowDown');
      await page.waitForTimeout(200);
      const afterDown = await page.evaluate(() => document.activeElement.getAttribute('role'));
      if (afterDown !== 'treeitem') issues.push('ArrowDown did not move to next treeitem');
      
      // Test: ArrowUp should move to previous visible item
      await page.keyboard.press('ArrowUp');
      await page.waitForTimeout(200);
      const afterUp = await page.evaluate(() => document.activeElement.getAttribute('role'));
      if (afterUp !== 'treeitem') issues.push('ArrowUp did not move to previous treeitem');
      
      // Test: ArrowRight on collapsed item should expand
      const firstItem = await page.$('[role="treeitem"][aria-expanded="false"]');
      if (firstItem) {
        await firstItem.focus();
        await page.keyboard.press('ArrowRight');
        await page.waitForTimeout(200);
        const expanded = await page.evaluate(el => el.getAttribute('aria-expanded'), firstItem);
        if (expanded !== 'true') issues.push('ArrowRight did not expand collapsed treeitem');
      }
      
      // Test: ArrowRight on expanded item should move to first child
      const expandedItem = await page.$('[role="treeitem"][aria-expanded="true"]');
      if (expandedItem) {
        await expandedItem.focus();
        await page.keyboard.press('ArrowRight');
        await page.waitForTimeout(200);
        const childRole = await page.evaluate(() => document.activeElement.getAttribute('role'));
        if (childRole !== 'treeitem') issues.push('ArrowRight on expanded item did not move to child');
      }
      
      // Test: ArrowLeft on expanded item should collapse
      const expandableItem = await page.$('[role="treeitem"][aria-expanded="true"]');
      if (expandableItem) {
        await expandableItem.focus();
        await page.keyboard.press('ArrowLeft');
        await page.waitForTimeout(200);
        const collapsed = await page.evaluate(el => el.getAttribute('aria-expanded'), expandableItem);
        if (collapsed !== 'false') issues.push('ArrowLeft did not collapse expanded treeitem');
      }
      
      // Test: Home should move to first item
      await page.keyboard.press('Home');
      await page.waitForTimeout(200);
      const homeRole = await page.evaluate(() => document.activeElement.getAttribute('role'));
      if (homeRole !== 'treeitem') issues.push('Home did not move to first treeitem');
      
      // Test: End should move to last visible item
      await page.keyboard.press('End');
      await page.waitForTimeout(200);
      const endRole = await page.evaluate(() => document.activeElement.getAttribute('role'));
      if (endRole !== 'treeitem') issues.push('End did not move to last treeitem');
      
      return { issues, tested: true };
    }
  }
};

// ═══════════════════════════════════════════════════════════════
// MAIN AUDIT FUNCTION
// ═══════════════════════════════════════════════════════════════

async function runWidgetAudit(url) {
  console.log(`\n🧩 Widget ARIA Audit: ${url}`);
  
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  try {
    await page.goto(url, { waitUntil: 'networkidle', timeout: 30000 });
  } catch (e) {
    // Fallback to domcontentloaded
    await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 30000 });
  }
  
  const allResults = {};
  
  for (const [patternId, pattern] of Object.entries(WIDGET_PATTERNS)) {
    const widgets = await page.evaluate(pattern.detect);
    
    if (widgets.length === 0) {
      allResults[patternId] = { 
        name: pattern.name, 
        detected: false, 
        widgets: 0,
        ariaIssues: [],
        keyboardIssues: [],
        tested: false
      };
      console.log(`   ${pattern.name}: 0 détecté`);
      continue;
    }
    
    console.log(`   ${pattern.name}: ${widgets.length} détecté(s)`);
    
    // ARIA attribute tests
    const allAriaIssues = [];
    for (const widget of widgets) {
      const issues = pattern.ariaTests(widget);
      allAriaIssues.push(...issues);
    }
    
    // Keyboard interaction tests
    let keyboardResult = { issues: [], tested: false };
    try {
      keyboardResult = await pattern.keyboardTests(page);
    } catch (e) {
      keyboardResult = { issues: [`Keyboard test error: ${e.message}`], tested: false };
    }
    
    const passed = allAriaIssues.length === 0 && keyboardResult.issues.length === 0;
    
    allResults[patternId] = {
      name: pattern.name,
      detected: true,
      widgets: widgets.length,
      widgetsDetail: widgets,
      ariaIssues: allAriaIssues,
      keyboardIssues: keyboardResult.issues,
      keyboardTested: keyboardResult.tested,
      passed
    };
    
    const status = passed ? '✅' : '❌';
    console.log(`     ${status} ARIA: ${allAriaIssues.length === 0 ? 'OK' : allAriaIssues.length + ' issue(s)'}`);
    console.log(`     ${status} Keyboard: ${keyboardResult.issues.length === 0 ? 'OK' : keyboardResult.issues.length + ' issue(s)'}`);
  }
  
  await browser.close();
  
  // Summary
  let totalAriaIssues = 0;
  let totalKeyboardIssues = 0;
  let totalWidgets = 0;
  
  for (const result of Object.values(allResults)) {
    if (result.detected) {
      totalWidgets += result.widgets;
      totalAriaIssues += result.ariaIssues.length;
      totalKeyboardIssues += result.keyboardIssues.length;
    }
  }
  
  console.log(`\n   📊 Total: ${totalWidgets} widget(s), ${totalAriaIssues} ARIA issue(s), ${totalKeyboardIssues} keyboard issue(s)`);
  
  return allResults;
}

module.exports = { runWidgetAudit, WIDGET_PATTERNS };

if (require.main === module) {
  const url = process.argv[2] || 'https://example.com';
  runWidgetAudit(url)
    .then(() => console.log('\n✅ Widget audit complete'))
    .catch(console.error);
}
