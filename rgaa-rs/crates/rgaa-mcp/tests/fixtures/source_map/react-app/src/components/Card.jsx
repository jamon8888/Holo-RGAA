import React from "react";

export function Card({ total }) {
  return (
    <section className="card">
      <button className="btn btn-primary">Détails</button>
      <span className="card-total">{total}</span>
    </section>
  );
}
