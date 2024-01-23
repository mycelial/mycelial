import * as React from 'react';
import FormControlLabel from '@mui/material/FormControlLabel';
import TextField from '@mui/material/TextField';
import { toTitleCase } from '../utils';
import Box from '@mui/material/Box';
import { fieldNames } from '../utils/constants';
import CustomSwitch from './CustomSwitch';

type NodeFormFieldProps = {
  key: string;
  formik: any;
};
const renderNodeFormField = ({ key, formik }: NodeFormFieldProps) => {
  console.log({ key });
  console.log({ formik });

  const fieldType: string = fieldNames[key] as string;
  switch (fieldType) {
    case 'number':
    case 'text':
      return (
        <Box my={2} key={key}>
          <TextField
            label={`${toTitleCase(key)}`}
            InputLabelProps={{ shrink: true }}
            name={key}
            key={key}
            type={fieldType}
            fullWidth
            sx={{
              '& .MuiFormLabel-root.MuiInputLabel-root': { color: '#05386B' },
            }}
            variant="filled"
            margin="dense"
            value={formik.values[key]}
            onChange={formik.handleChange}
            onBlur={formik.handleBlur}
            error={formik.touched[key] && Boolean(formik.errors[key])}
            helperText={formik.touched[key] && formik.errors[key]}
          />
        </Box>
      );
    case 'boolean':
      return (
        <Box p={2} my={1} ml="6px" key={key}>
          <FormControlLabel
            checked={formik.values[key]}
            name={key}
            control={<CustomSwitch />}
            label={toTitleCase(key)}
            onChange={formik.handleChange}
          />
        </Box>
      );
  }
};

export default renderNodeFormField;
