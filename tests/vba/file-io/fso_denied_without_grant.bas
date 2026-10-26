' INV: obj.filesystemobject
' SOURCE: derived the host policy denies file access by default; creating the object needs only the COM grant, using a file needs the file grants
' ERROR: 70
' GRANT: com-create
Public Sub Main()
    Dim fs As Object
    Set fs = CreateObject("Scripting.FileSystemObject")
    fs.CreateTextFile "/x.txt"
End Sub
