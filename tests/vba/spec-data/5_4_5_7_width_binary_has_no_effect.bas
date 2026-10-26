' INV: vbal.5.4.5.7, stmt.width
' SOURCE: spec ms-vbal 5.4.5.7 Width has no effect on Binary and Random files
' EXPECT: err=0
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/w.bin" For Binary As #1
    Open "/t/w.dat" For Random As #2 Len = 4
    On Error Resume Next
    Width #1, 10
    Width #2, 10
    Debug.Print "err=" & CStr(Err.Number)
End Sub
