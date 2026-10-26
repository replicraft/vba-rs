' INV: vbal.5.4.5.1, stmt.get
' SOURCE: spec ms-vbal 5.4.5.1 Get # is not valid on a file opened for Output
' EXPECT: err=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim n As Long
    Open "/t/m.txt" For Output As #1
    On Error Resume Next
    Get #1, 1, n
    Debug.Print "err=" & CStr(Err.Number <> 0)
End Sub
