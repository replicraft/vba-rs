' INV: vbal.6.1.2.5.1.2, func.dir
' SOURCE: spec ms-vbal 6.1.2.5.1.2 Dir returns zero-length string when PathName is not found
' EXPECT: none=[]
' EXPECT: type=String
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Debug.Print "none=[" & Dir("/nothing_here.txt") & "]"
    Debug.Print "type=" & TypeName(Dir("/nothing_here.txt"))
End Sub
