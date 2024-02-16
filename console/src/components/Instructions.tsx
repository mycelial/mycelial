import React, { SyntheticEvent, MouseEvent, useState } from "react";
import { Paper, Tabs, Tab, Typography, Box } from "@mui/material";

function Instructions(props) {
  let token = props.token;
  return (
    <Paper
      sx={{
        my: 2,
        p: 4,
        width: "70%",
        backgroundColor: '#a5d6a7', // todo: change to theme
        color: "black", // todo: change to theme
        boxShadow: 3,
        borderRadius: 1,
      }}
      elevation={3}
    >
      <p>
        To add your local daemon to this mycelial network, simply install the
        daemon using the instructions found{" "}
        <a
          href="https://docs.mycelial.com/getting-started/CLI/"
          target="_blank"
        >
          here
        </a>{" "}
        and copied below.
      </p>

      <CommandLineTabs token={token} />
    </Paper>
  );
}

function TabPanel(props) {
  const { children, value, index, ...other } = props;

  return (
    <div
      role="tabpanel"
      hidden={value !== index}
      id={`simple-tabpanel-${index}`}
      aria-labelledby={`simple-tab-${index}`}
      {...other}
    >
      {value === index && <Box sx={{ p: 3 }}>{children}</Box>}
    </div>
  );
}

function a11yProps(index) {
  return {
    id: `simple-tab-${index}`,
    "aria-controls": `simple-tabpanel-${index}`,
  };
}

function CommandLineTabs(props) {
  let token = props.token;
  const [value, setValue] = React.useState(0);

  const handleChange = (event, newValue) => {
    setValue(newValue);
  };

  // endpoint for the window
  const endpoint = window.location.origin;

  return (
    <Paper
      sx={{
        p: 2,
        backgroundColor: '#f6f6f6', // todo: change to theme
        color: "success.light", // todo: change to theme
        boxShadow: 1,
        borderRadius: 1,
        width: "100%",
      }}
      elevation={3}
    >
      <Tabs
        value={value}
        onChange={handleChange}
        aria-label="OS Tabs"
      >
        <Tab label="Mac" {...a11yProps(0)} />
        <Tab label="Linux" {...a11yProps(1)} />
      </Tabs>
      <TabPanel value={value} index={0}>
        {/* Mac Instructions */}
        <Typography
          variant="body1"
          component="pre"
          sx={{
            fontFamily: "monospace",
            whiteSpace: "pre-wrap",
            wordBreak: "break-word",
          }}
        >
          $ brew install mycelial/tap/mycelial{"\n"}$ mycelial init --daemon
          --endpoint "{endpoint}" --token "{token}"{"\n"}$ mycelial start
          --daemon
        </Typography>
      </TabPanel>
      <TabPanel value={value} index={1}>
        {/* Linux Instructions */}
        <Typography
          variant="body1"
          component="pre"
          sx={{
            fontFamily: "monospace",
            whiteSpace: "pre-wrap",
            wordBreak: "break-word",
          }}
        >
          Installation instructions can be found{" "}
          <a
            href="https://docs.mycelial.com/getting-started/CLI/"
            target="_blank"
          >
            here
          </a>
          .
        </Typography>
      </TabPanel>
    </Paper>
  );
}

export default Instructions;
