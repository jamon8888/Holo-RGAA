import React, { useState, useCallback } from "react";
import { Card, Stack } from "../ui/primitives";
import { useCatalogue } from "../hooks/useCatalogue";
import type { Product, Facet } from "../types";

interface Props {
  products: Product[];
  facets: Facet[];
  onSelect: (id: string) => void;
}

export function CatalogueGrid({ products, facets, onSelect }: Props) {
  const [query, setQuery] = useState("");
  const [openFacet, setOpenFacet] = useState<string | null>(null);
  const { loading, refresh } = useCatalogue(query);

  const toggleFacet = useCallback(
    (id: string) => setOpenFacet((current) => (current === id ? null : id)),
    [],
  );

  return (
    <section className="catalogue" aria-labelledby="catalogue-heading">
      <h2 id="catalogue-heading">Catalogue</h2>

      <form className="catalogue__search" onSubmit={(e) => e.preventDefault()}>
        <label htmlFor="catalogue-query">Rechercher un produit</label>
        <input
          id="catalogue-query"
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <input type="text" name="reference" placeholder="Référence" />
        <select name="tri">
          <option value="prix">Prix</option>
          <option value="nom">Nom</option>
        </select>
        <label>
          Uniquement en stock
          <input type="checkbox" name="stock" />
        </label>
        <input type="hidden" name="csrf" value="tok" />
        <button type="submit">Filtrer</button>
        <button type="button" onClick={refresh}>
          <img src="/icons/refresh.svg" alt="" />
        </button>
      </form>

      <nav className="catalogue__facets">
        <ul>
          {facets.map((facet) => (
            <li key={facet.id}>
              <button onClick={() => toggleFacet(facet.id)}>{facet.label}</button>
              <a href={`/facet/${facet.id}`}>{facet.label}</a>
            </li>
          ))}
        </ul>
      </nav>

      <div className="catalogue__grid">
        {products.map((product) => (
          <Card key={product.id} onClick={() => onSelect(product.id)}>
            <img src={product.image} alt={product.name} width={240} height={180} />
            <img src={product.badge} className="badge" />
            <h3>{product.name}</h3>
            <p className="price">{product.price} €</p>
            <a href={`/produit/${product.slug}`}>
              <img src="/icons/chevron.svg" />
            </a>
            <a href={`/panier/ajouter/${product.id}`} aria-label={`Ajouter ${product.name}`}>
              <img src="/icons/cart.svg" alt="" />
            </a>
            <button className="icon-only" onClick={() => onSelect(product.id)} />
          </Card>
        ))}
      </div>

      {loading && <p role="status">Chargement…</p>}

      <footer className="catalogue__footer">
        <a href="/mentions-legales">Mentions légales</a>
        <a href="/rss"><img src="/icons/rss.svg" alt="Flux RSS" /></a>
        <a href="/twitter"></a>
      </footer>
    </section>
  );
}
