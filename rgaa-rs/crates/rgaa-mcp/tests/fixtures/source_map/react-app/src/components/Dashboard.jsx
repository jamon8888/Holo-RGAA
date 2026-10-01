import React from "react";
import { Card } from "./Card";

export function Dashboard({ revenue, items }) {
  return (
    <main id="dashboard">
      <h1 className="page-title">Tableau de bord</h1>
      <img
        className="kpi-chart"
        src="/static/charts/revenue-2024.png"
        alt="Chiffre d'affaires par trimestre"
      />
      <button className="btn btn-primary" id="export-csv">
        Exporter
      </button>
      <ul className="item-list">
        {items.map((item) => (
          <li key={item.id} className="item-row">
            <img className="item-thumb" src={item.thumbnail} alt={item.label} />
          </li>
        ))}
      </ul>
      <Card total={revenue} />
    </main>
  );
}
