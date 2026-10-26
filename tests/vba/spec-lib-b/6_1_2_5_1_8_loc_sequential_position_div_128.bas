' INV: vbal.6.1.2.5.1.8, func.loc
' SOURCE: spec ms-vbal 6.1.2.5.1.8 Loc of a sequential file is the current byte position divided by 128
' EXPECT: out=2
' EXPECT: in=2
' EXPECT: seek=257
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/s.txt" For Output As #1
    Print #1, String(254, "x")
    Debug.Print "out=" & CStr(Loc(1))
    Close #1
    Open "/t/s.txt" For Input As #1
    Line Input #1, s
    Debug.Print "in=" & CStr(Loc(1))
    Debug.Print "seek=" & CStr(Seek(1))
    Close #1
End Sub
