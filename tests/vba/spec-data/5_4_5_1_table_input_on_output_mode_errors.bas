' INV: vbal.5.4.5.1, stmt.input
' SOURCE: spec ms-vbal 5.4.5.1 Input # is not valid on a file opened for Output
' EXPECT: err=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/m.txt" For Output As #1
    On Error Resume Next
    Input #1, s
    Debug.Print "err=" & CStr(Err.Number <> 0)
End Sub
