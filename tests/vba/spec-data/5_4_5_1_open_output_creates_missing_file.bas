' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 missing file is created unless mode is Input
' EXPECT: before=[]
' EXPECT: after=[n.txt]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Debug.Print "before=[" & Dir("/t/n.txt") & "]"
    Open "/t/n.txt" For Output As #1
    Close #1
    Debug.Print "after=[" & Dir("/t/n.txt") & "]"
End Sub
