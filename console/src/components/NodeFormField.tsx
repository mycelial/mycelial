import * as React from "react";
import FormControlLabel from "@mui/material/FormControlLabel";
import TextField from "@mui/material/TextField";
import { toTitleCase } from "../utils";
import Box from "@mui/material/Box";
import { fieldNames } from "../utils/constants";
import CustomSwitch from "./CustomSwitch";
import { useTheme } from "@mui/material/styles";
import { Icon } from "@mui/material";
import { Warning, Info } from "@mui/icons-material";

type NodeFormFieldProps = {
  key: string;
  formik: any;
  original: any;
};
const renderNodeFormField = ({ key, formik, original }: NodeFormFieldProps) => {
  let fieldType: string = fieldNames[key] as string;
  if (fieldType === undefined) {
    console.warn(`No field type found for ${key}`);
    fieldType = "text";
  }

  const theme = useTheme();

  const hasChanged = original === undefined || (
    original[key] !== formik.values[key] || original[key] === undefined);

  switch (fieldType) {
    case "number":
    case "text":
      return (
        <Box my={2} key={key} display="flex">
          <TextField
            label={`${toTitleCase(key)}`}
            InputLabelProps={{ shrink: true }}
            name={key}
            key={key}
            type={fieldType}
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
          {hasChanged && (
            <Icon
              sx={{
                marginLeft: "-2rem",
                marginTop: "1rem",
                zIndex: 100,
                color: theme.palette.warning.main,
              }}
              title={`Changed from "${original ? original[key] : ("")}"`}
            >
              <Info />
            </Icon>
          )}
        </Box>
      );
    case "boolean":
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
