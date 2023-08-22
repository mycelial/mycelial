"use client";

import React, { Fragment, useEffect, useState } from "react";
import styled from '@emotion/styled';
import { Listbox, Transition } from "@headlessui/react";
import { CheckIcon, ChevronUpDownIcon } from "@heroicons/react/20/solid";
export { MultiSelect, NativeSelect } from "@mantine/core";
import {
  createStyles,
  rem,
  Flex,
  NativeSelect
} from "@/components/core";

const useStyles = createStyles((theme) => ({
  customFlexRow: {
    paddingLeft: '1em', 
    paddingRight: '1em', 
  },
  customInputLabel: {
    cursor: "inherit",
  },
  customInputText: {
    backgroundColor: theme.colors.night[0],
    color: theme.colors.stem[0],
    borderColor: "transparent",
    borderRadius: rem(2), 
    marginTop: rem(2),
  },
  input: {
    backgroundColor: theme.colors.stem[1],
    color: theme.colors.night[1],
  },
  label: {
    color: theme.colors.stem[0]
  },
  root: {
    padding: rem(5),
  }
  
}));

const getRandomString = () => {
  return String(Date.now().toString(32) + Math.random().toString(16)).replace(
    /\./g,
    ""
  );
};

const getId = (name: string) => `${name}_${getRandomString()}`;

interface TextInputProps {
  label: string;
  name: string;
  type?: string;
  placeholder: string;
  defaultValue: string;
  onChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
}

// TODO: replace with mantine component
export const TextInput: React.FC<TextInputProps> = (props) => {
  const id = React.useRef<string>("");
  const { classes } = useStyles();
  useEffect(() => {
    id.current = getId(props.name);
  }, [props.name]);

  return (
    <div className={classes.customFlexRow}>
      <label
        className={classes.customInputLabel}
        htmlFor={id.current}
      >
        {props.label}
      </label>
      <div className="">
        <input
          type={props.type || "text"}
          name={props.name}
          id={id.current}
          placeholder={props.placeholder}
          defaultValue={props.defaultValue}
          onChange={props.onChange}
          className={classes.customInputText}
        />
      </div>
    </div>
  );
};

interface TextAreaProps {
  label: string;
  name: string;
  type?: string;
  placeholder?: string;
  defaultValue?: string;
  onChange: (event: React.ChangeEvent<HTMLTextAreaElement>) => void;
}

export const TextArea: React.FC<TextAreaProps> = (props) => {
  const id = React.useRef<string>("");
  useEffect(() => {
    id.current = getId(props.name);
  }, [props.name]);

  return (
    <div>
      <label
        style={{ cursor: "inherit" }}
        htmlFor={id.current}
      >
        {props.label}
      </label>
      <div className="">
        <textarea
          rows={4}
          name={props.name}
          id={id.current}
          defaultValue={props.defaultValue || ""}
          placeholder={props.placeholder || ""}
          onChange={props.onChange}
        />
      </div>
    </div>
  );
};

interface SelectProps {
  label: string;
  name: string;
  type?: string;
  placeholder?: string;
  defaultValue?: string;
  onChange: (value: string) => void;
  options: string[];
}


export const Select: React.FC<SelectProps> = (props) => {
  const id = React.useRef<string>("");
  const { classes } = useStyles();


  useEffect(() => {
    id.current = getId(props.name);
  }, [props.name]);

  const [selected, setSelected] = useState(props.defaultValue);
  useEffect(() => {
    props.onChange(selected || "");
  }, [selected]);



  return (
    <NativeSelect
     classNames={
      {
        input: classes.input,
        label: classes.label,
        root: classes.root,
      }
     }
     // TODO: Implement no drag when this selector is open.
     label={props.label}
     data={props.options}
     value={selected}
   />
  )
};
