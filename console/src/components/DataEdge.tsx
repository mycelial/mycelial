import { Box, Button } from '@mui/material';
import React, { useCallback } from 'react';
import useFlowStore, { selector } from '../stores/flowStore';
import { BaseEdge, EdgeLabelRenderer, EdgeProps, getBezierPath, useReactFlow } from 'reactflow';

export default function DataEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  markerEnd,
  selected,
}: EdgeProps) {
  const rf = useReactFlow();
  const xEqual = sourceX === targetX;
  const yEqual = sourceY === targetY;

  const [edgePath, labelX, labelY] = getBezierPath({
    // we need this little hack in order to display the gradient for a straight line
    sourceX: xEqual ? sourceX + 0.0001 : sourceX,
    sourceY: yEqual ? sourceY + 0.0001 : sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });
  const { edges, setEdges } = useFlowStore(selector);

  const onRemove = useCallback(() => {
    const updatedEdges = edges.filter((edge) => edge.id !== id);
    setEdges(updatedEdges);
    // following action triggers onEdgeChange, which updates edgesToBeDeleted
    rf.deleteElements({ edges: [{ id }] });
  }, [id]);
  return (
    <>
      <BaseEdge
        path={edgePath}
        markerEnd={markerEnd}
        interactionWidth={25}
        style={{
          stroke: 'url(#gradient)',
          strokeLinecap: 'round',
          strokeLinejoin: 'round',
          strokeWidth: `${selected ? '3px' : '2px'}`,
        }}
      />
      <EdgeLabelRenderer>
        <Box
          sx={{
            position: 'absolute',
            transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
            fontSize: 12,
            // everything inside EdgeLabelRenderer has no pointer events by default
            // bc we have button, set pointer-events: all
            pointerEvents: 'all',
          }}
          className="nodrag nopan"
        >
          <button
            className="edgebutton"
            style={{
              width: '20px',
              height: '20px',
              background: '#f6f6f6',
              border: ' 1px solid #f6f6f6',
              cursor: 'pointer',
              borderRadius: '50%',
              fontSize: '12px',
              lineHeight: 1,
              color: 'red',
            }}
            onClick={onRemove}
          >
            ×
          </button>
        </Box>
      </EdgeLabelRenderer>
    </>
  );
}
