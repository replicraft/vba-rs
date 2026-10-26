' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 missing file is created unless mode is Input
' EXPECT: [b.bin]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/b.bin" For Binary As #1
    Close #1
    Debug.Print "[" & Dir("/t/b.bin") & "]"
End Sub
