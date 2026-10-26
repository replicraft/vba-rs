' INV: obj.filesystemobject, obj.textstream
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/opentextfile-method
' EXPECT: Hello world!
' EXPECT: Hello world!more
' EXPECT: new
' EXPECT: 2
' EXPECT: ab
' EXPECT: Unicode ok
' EXPECT: 0
' GRANT: com-create
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Const ForReading = 1, ForWriting = 2, ForAppending = 8
    Dim fs As Object, f As Object
    Set fs = CreateObject("Scripting.FileSystemObject")
    ' Create when missing (ForAppending, Create:=True, ASCII).
    Set f = fs.OpenTextFile("/t.txt", ForAppending, True, 0)
    f.Write "Hello world!"
    f.Close
    Debug.Print fs.OpenTextFile("/t.txt", ForReading).ReadAll
    Set f = fs.OpenTextFile("/t.txt", ForAppending)
    f.Write "more"
    f.Close
    Debug.Print fs.OpenTextFile("/t.txt").ReadAll
    ' ForWriting replaces the contents.
    Set f = fs.OpenTextFile("/t.txt", ForWriting)
    f.Write "new"
    f.Close
    Debug.Print fs.OpenTextFile("/t.txt").ReadAll
    ' Read, Skip, Column.
    Set f = fs.OpenTextFile("/t.txt", ForWriting)
    f.Write "abcdef"
    f.Close
    Set f = fs.OpenTextFile("/t.txt")
    f.Skip 1
    Debug.Print f.Column & ""
    f.Close
    Set f = fs.OpenTextFile("/t.txt")
    Debug.Print f.Read(2)
    f.Close
    ' Unicode (UTF-16LE with a byte order mark).
    Set f = fs.CreateTextFile("/u.txt", True, True)
    f.Write "Unicode ok"
    f.Close
    Debug.Print fs.OpenTextFile("/u.txt", ForReading, False, -1).ReadAll
    Debug.Print fs.GetFile("/u.txt").Size - 2 * Len("Unicode ok") - 2 & ""
End Sub
