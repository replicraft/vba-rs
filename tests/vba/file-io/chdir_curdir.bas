' INV: stmt.chdir, func.curdir, stmt.mkdir
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/chdir-statement
' EXPECT: /work
' EXPECT: rel.txt
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/work"
    ChDir "/work"
    Debug.Print CurDir
    Open "rel.txt" For Output As #1: Close #1
    Debug.Print Dir("/work/rel.txt")
End Sub
