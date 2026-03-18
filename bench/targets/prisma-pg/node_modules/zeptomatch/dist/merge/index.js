/* MAIN */
const merge = (res) => {
    const source = res.map(re => re.source).join('|') || '$^';
    const flags = res[0]?.flags;
    return new RegExp(source, flags);
};
/* EXPORT */
export default merge;
