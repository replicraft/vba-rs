' INV: vbal.6.1.2.5.2.1, stmt.chdir, func.curdir
' SOURCE: spec ms-vbal 6.1.2.5.2.1 ChDir changes the current directory (relative file names follow)
' EXPECT: rel=True
' EXPECT: restored=False
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer
    MkDir "/cd1"
    ChDir "/cd1"
    f = FreeFile
    Open "rel.txt" For Output As #f
    Close #f
    Debug.Print "rel=" & CStr(Dir("/cd1/rel.txt") = "rel.txt")
    ChDir "/"
    Debug.Print "restored=" & CStr(Dir("rel.txt") = "rel.txt")
End Sub
