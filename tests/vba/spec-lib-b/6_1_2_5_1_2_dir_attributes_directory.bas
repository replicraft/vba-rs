' INV: vbal.6.1.2.5.1.2, func.dir
' SOURCE: spec ms-vbal 6.1.2.5.1.2 Dir attributes: omitted means no attributes; vbDirectory includes directories
' EXPECT: plain=[]
' EXPECT: withdir=sub
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/dd"
    MkDir "/dd/sub"
    Debug.Print "plain=[" & Dir("/dd/sub") & "]"
    Debug.Print "withdir=" & Dir("/dd/sub", vbDirectory)
End Sub
