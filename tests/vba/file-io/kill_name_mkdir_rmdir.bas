' INV: stmt.kill, stmt.name, stmt.mkdir, stmt.rmdir, func.dir
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/kill-statement
' EXPECT: b.txtTrue
' EXPECT: True
' EXPECT: True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/d"
    Open "/d/a.txt" For Output As #1
    Close #1
    Name "/d/a.txt" As "/d/b.txt"
    Debug.Print Dir("/d/b.txt"); Dir("/d/a.txt") = ""
    Kill "/d/b.txt"
    Debug.Print Dir("/d/*.txt") = ""
    RmDir "/d"
    Debug.Print Dir("/d", vbDirectory) = ""
End Sub
