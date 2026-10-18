using DocumentFormat.OpenXml;
using DocumentFormat.OpenXml.Packaging;
using DocumentFormat.OpenXml.Validation;
var validator=new OpenXmlValidator(FileFormatVersions.Microsoft365);
var count=0;
foreach(var path in args){using var doc=PresentationDocument.Open(path,false);var errors=validator.Validate(doc).ToList();count+=errors.Count;Console.WriteLine($"{path}: {errors.Count} validation errors");foreach(var e in errors)Console.WriteLine($"{e.Part?.Uri} {e.Path?.XPath}: {e.Description}");}
Environment.ExitCode=count==0?0:1;
