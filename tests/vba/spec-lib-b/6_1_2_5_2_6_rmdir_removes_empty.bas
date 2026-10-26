' INV: vbal.6.1.2.5.2.6, stmt.rmdir
' SOURCE: spec ms-vbal 6.1.2.5.2.6 RmDir removes an existing directory
' EXPECT: before=gone
' EXPECT: after=[]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/gone"
    Debug.Print "before=" & Dir("/gone", vbDirectory)
    RmDir "/gone"
    Debug.Print "after=[" & Dir("/gone", vbDirectory) & "]"
End Sub
