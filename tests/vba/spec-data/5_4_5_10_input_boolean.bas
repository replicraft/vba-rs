' INV: vbal.5.4.5.10, stmt.input
' SOURCE: spec ms-vbal 5.4.5.10 Boolean is true only for #TRUE#
' EXPECT: t=True
' EXPECT: f=False
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim t As Boolean, f As Boolean
    f = True
    Open "/t/i.txt" For Output As #1
    Write #1, True
    Close #1
    Open "/t/i.txt" For Input As #1
    Input #1, t
    Close #1
    Open "/t/i2.txt" For Output As #1
    Write #1, False
    Close #1
    Open "/t/i2.txt" For Input As #1
    Input #1, f
    Close #1
    Debug.Print "t=" & CStr(t)
    Debug.Print "f=" & CStr(f)
End Sub
