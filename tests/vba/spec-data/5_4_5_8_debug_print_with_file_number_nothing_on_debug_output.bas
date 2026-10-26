' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 revision 3.0 grammar: file-number form writes to the file, not to Debug output
' EXPECT: only
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/p.txt" For Output As #1
    Debug.Print #1, "notshown"
    Close #1
    Debug.Print "only"
End Sub
