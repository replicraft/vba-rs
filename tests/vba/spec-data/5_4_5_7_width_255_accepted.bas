' INV: vbal.5.4.5.7, stmt.width
' SOURCE: spec ms-vbal 5.4.5.7 line width 255 is within range
' EXPECT: err=0
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/w.txt" For Output As #1
    On Error Resume Next
    Width #1, 255
    Width #1, 0
    Debug.Print "err=" & CStr(Err.Number)
End Sub
