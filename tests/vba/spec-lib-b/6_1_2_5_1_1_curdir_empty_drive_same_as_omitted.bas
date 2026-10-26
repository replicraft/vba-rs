' INV: vbal.6.1.2.5.1.1, func.curdir
' SOURCE: spec ms-vbal 6.1.2.5.1.1 CurDir with zero-length Drive equals CurDir with Drive unspecified
' EXPECT: same=True
' EXPECT: type=String
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Debug.Print "same=" & CStr(CurDir("") = CurDir)
    Debug.Print "type=" & TypeName(CurDir$)
End Sub
