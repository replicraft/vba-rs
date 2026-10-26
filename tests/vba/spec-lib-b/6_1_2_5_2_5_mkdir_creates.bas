' INV: vbal.6.1.2.5.2.5, stmt.mkdir
' SOURCE: spec ms-vbal 6.1.2.5.2.5 MkDir creates a new directory
' EXPECT: before=[]
' EXPECT: after=made
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Debug.Print "before=[" & Dir("/made", vbDirectory) & "]"
    MkDir "/made"
    Debug.Print "after=" & Dir("/made", vbDirectory)
End Sub
