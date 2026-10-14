"use client";

import "../../iso/iso.css";
import "./sorting-machine.css";
import { Plate } from "../../iso/react/draw";
import { FIRST_READOUT, Machine } from "./machine";

// Hero figure for the landing page: a mail-sorting machine. Fills its parent's width.
export function SortingMachine() {
  return (
    <div className="iso sm">
      <Plate fig="Fig 1" title="Sending queue" hint="Drag the knob" readout={FIRST_READOUT}>
        <Machine />
      </Plate>
    </div>
  );
}
