' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 Len 32767 is the upper limit of the valid range
' EXPECT: ok
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/len.dat" For Random As #1 Len = 32767
    Close #1
    Debug.Print "ok"
End Sub
