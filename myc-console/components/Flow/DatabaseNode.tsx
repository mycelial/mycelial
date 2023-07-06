import { memo, FC } from 'react';
import { Handle, Position, NodeProps, NodeResizer } from 'reactflow';
// import * as Select from '@radix-ui/react-select';

import { Select } from '@/components/inputs';


const DatabaseNode: FC<NodeProps> = ({ id, data }) => {
  return (
    <div>
      <Select
        label="Database"
        placeholder="Pick one"
        searchable
        nothingFound="No options"
        data={['Sqlite', 'PostgreSQL', 'Snowflake']}
      />
    </div>
  );
}

export default memo(DatabaseNode);