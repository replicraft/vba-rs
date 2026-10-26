' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 no mode clause means Random
' EXPECT: 77 88
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim a As Long, b As Long
    Open "/t/r.dat" As #1 Len = 4
    a = 77
    Put #1, 1, a
    a = 88
    Put #1, 2, a
    Get #1, 1, a
    Get #1, 2, b
    Close #1
    Debug.Print CStr(a) & " " & CStr(b)
End Sub
