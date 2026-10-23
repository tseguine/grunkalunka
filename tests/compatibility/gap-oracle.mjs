// Only explicitly curated gap references opt into this isolated child process.
const module = await import(`data:text/javascript;base64,${process.argv[2]}`);
const value = await module[process.argv[4] ?? 'run'](...JSON.parse(process.argv[3]));
if (typeof value !== 'number') throw new Error('reference result must be a primitive number');
const view = new DataView(new ArrayBuffer(8));
view.setFloat64(0, value, false);
process.stdout.write(view.getBigUint64(0, false).toString(16).padStart(16, '0'));
