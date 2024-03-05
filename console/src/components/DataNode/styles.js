const styles = {
  node: {
    background: '#FFF',
    color: '#05386B',
    padding: 2,
    display: 'flex',
    flexDirection: 'column',
    fontSize: '.8rem',
    fontWeight: '550',
    minHeight: '100px',
    width: '200px',
  },
  gradient: {
    border: '1.5px solid transparent',
    borderRadius: '4px',
    backgroundImage: `linear-gradient(#fff, #fff), linear-gradient(to right, #a5d6a7 5%, #97b398 40%, #52776b 69%, #3a554c 95%)`,
    backgroundOrigin: 'border-box',
    backgroundClip: 'content-box, border-box',
    '& .selected': {
      border: '3px solid transparent',
    },
  },
  handle: {
    width: 12,
    height: 12,
  },
};

export default styles;
