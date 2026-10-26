' INV: vbal.5.4.5.3, stmt.seek
' SOURCE: spec ms-vbal 5.4.5.3 new position 0 or negative raises an error
' EXPECT: zero=True
' EXPECT: neg=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/s.bin" For Binary As #1
    On Error Resume Next
    Seek #1, 0
    Debug.Print "zero=" & CStr(Err.Number <> 0)
    Err.Clear
    Seek #1, -3
    Debug.Print "neg=" & CStr(Err.Number <> 0)
End Sub
