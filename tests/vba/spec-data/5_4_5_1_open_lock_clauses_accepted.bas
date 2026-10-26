' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 lock clause forms
' EXPECT: ok
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/l1" For Binary Access Read Write Shared As #1
    Close #1
    Open "/t/l2" For Binary Lock Read As #1
    Close #1
    Open "/t/l3" For Binary Lock Write As #1
    Close #1
    Open "/t/l4" For Binary Lock Read Write As #1
    Close #1
    Debug.Print "ok"
End Sub
