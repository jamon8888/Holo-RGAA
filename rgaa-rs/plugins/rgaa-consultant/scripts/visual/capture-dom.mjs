// Helpers DOM de capture.mjs. Ils restent séparés pour tester le comportement
// des propositions sans exécuter tout le flux de capture.

export const selectorOf = (target) => {
  const selector = Array.isArray(target) ? target.at(-1) : target;
  return typeof selector === 'string' ? selector : '';
};

// Surligne les éléments correspondant aux sélecteurs et pose une étiquette.
// Renvoie le rectangle (coordonnées document) englobant les cibles trouvées.
export async function highlight(page, selectors, label, color) {
  return page.evaluate(({ selectors, label, color }) => {
    let box = null;
    for (const sel of selectors) {
      if (typeof sel !== 'string' || sel.length === 0) continue;
      let els;
      try { els = document.querySelectorAll(sel); } catch { continue; }
      for (const el of els) {
        const style = getComputedStyle(el);
        if (style.display === 'none' || style.visibility === 'hidden') continue;
        const r = el.getBoundingClientRect();
        if (r.width === 0 && r.height === 0) continue;
        el.style.setProperty('outline', `4px solid ${color}`, 'important');
        el.style.setProperty('outline-offset', '2px', 'important');
        const x = r.left + scrollX, y = r.top + scrollY;
        const tag = document.createElement('div');
        tag.textContent = label;
        tag.setAttribute('data-rgaa-visual', '');
        Object.assign(tag.style, {
          position: 'absolute', left: `${x}px`, top: `${Math.max(0, y - 26)}px`,
          background: color, color: '#fff', font: 'bold 14px/20px sans-serif',
          padding: '2px 6px', zIndex: 2147483647, pointerEvents: 'none',
        });
        document.body.appendChild(tag);
        box = box
          ? { x1: Math.min(box.x1, x), y1: Math.min(box.y1, y - 26), x2: Math.max(box.x2, x + r.width), y2: Math.max(box.y2, y + r.height) }
          : { x1: x, y1: y - 26, x2: x + r.width, y2: y + r.height };
      }
    }
    return box;
  }, { selectors, label, color });
}

// Applique une proposition dans la page. Renvoie un sélecteur stable pour
// surligner ensuite les éléments modifiés, même si la proposition change le
// sélecteur d'origine ou remplace son outerHTML.
export async function apply(page, sel, proposal) {
  if (proposal.css) await page.addStyleTag({ content: proposal.css });
  return page.evaluate(({ sel, proposal }) => {
    let elements;
    try { elements = document.querySelectorAll(sel); } catch { return null; }
    if (elements.length === 0) return null;

    const urlAttributes = new Set([
      'action', 'background', 'cite', 'codebase', 'formaction', 'href', 'longdesc',
      'poster', 'src', 'xlink:href',
    ]);
    const isSafeUrl = (value) => {
      const normalized = value.trim().replace(/[\u0000-\u0020\u007f]+/g, '').toLowerCase();
      if (!normalized || normalized.startsWith('#') || normalized.startsWith('/') ||
          normalized.startsWith('./') || normalized.startsWith('../')) return true;
      const scheme = normalized.match(/^([a-z][a-z0-9+.-]*):/);
      if (!scheme) return true;
      if (['http', 'https', 'mailto', 'tel'].includes(scheme[1])) return true;
      return scheme[1] === 'data' && /^data:image\/(?:avif|gif|jpeg|png|webp);/.test(normalized);
    };
    const applyAttributes = (element) => {
      for (const [name, value] of Object.entries(proposal.attributes ?? {})) {
        if (value === null) {
          element.removeAttribute(name);
          continue;
        }
        const attributeName = name.toLowerCase();
        if (attributeName.startsWith('on') || attributeName === 'srcdoc' ||
            attributeName === 'ping' || attributeName === 'is') continue;
        if (urlAttributes.has(attributeName) && !isSafeUrl(String(value))) continue;
        element.setAttribute(name, String(value));
      }
    };

    if (proposal.html) {
      const template = document.createElement('template');
      template.innerHTML = proposal.html.trim();

      // La proposition est une entrée locale, mais reste une frontière de
      // confiance : retire le contenu actif avant insertion dans la page.
      const blockedElements = new Set([
        'applet', 'animate', 'base', 'embed', 'foreignobject', 'frame', 'frameset',
        'iframe', 'link', 'meta', 'object', 'portal', 'script', 'set', 'template',
      ]);
      for (const element of template.content.querySelectorAll('*')) {
        if (blockedElements.has(element.localName.toLowerCase())) element.remove();
      }

      for (const element of template.content.querySelectorAll('*')) {
        for (const attribute of Array.from(element.attributes)) {
          const name = attribute.name.toLowerCase();
          if (name.startsWith('on') || name === 'srcdoc' || name === 'ping' || name === 'is' ||
              (urlAttributes.has(name) && !isSafeUrl(attribute.value))) {
            element.removeAttribute(attribute.name);
          }
        }
      }

      const roots = Array.from(template.content.children);
      if (roots.length === 0) return null;
      for (const element of elements) {
        const fragment = template.content.cloneNode(true);
        const replacements = Array.from(fragment.children);
        applyAttributes(replacements[0]);
        for (const replacement of replacements) replacement.setAttribute('data-rgaa-applied', '');
        element.replaceWith(fragment);
      }
    } else {
      for (const element of elements) {
        applyAttributes(element);
        element.setAttribute('data-rgaa-applied', '');
      }
    }
    return '[data-rgaa-applied]';
  }, { sel, proposal });
}
