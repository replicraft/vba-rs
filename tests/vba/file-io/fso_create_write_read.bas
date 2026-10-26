' INV: obj.filesystemobject, obj.textstream
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/createtextfile-method
' EXPECT: This is a test.
' EXPECT: second line
' EXPECT: True
' EXPECT: 2
' GRANT: com-create
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim fs As Object, a As Object
    Set fs = CreateObject("Scripting.FileSystemObject")
    Set a = fs.CreateTextFile("/testfile.txt", True)
    a.WriteLine "This is a test."
    a.WriteLine "second line"
    a.Close
    Set a = fs.OpenTextFile("/testfile.txt", 1)
    Debug.Print a.ReadLine
    Debug.Print a.ReadLine
    Debug.Print a.AtEndOfStream
    Debug.Print CStr(a.Line - 1)
    a.Close
End Sub
