' INV: vbal.6.1.2.5.2.3, stmt.filecopy
' SOURCE: spec ms-vbal 6.1.2.5.2.3 FileCopy raises an error if the Source file is currently open
' EXPECT: raised=True
' EXPECT: notcopied=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer
    f = FreeFile
    Open "/so.bin" For Binary As #f
    Put #f, , CByte(1)
    On Error Resume Next
    FileCopy "/so.bin", "/so2.bin"
    Debug.Print "raised=" & CStr(Err.Number <> 0)
    On Error GoTo 0
    Close #f
    Debug.Print "notcopied=" & CStr(Dir("/so2.bin") = "")
End Sub
