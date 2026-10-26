' INV: vbal.6.1.2.11.1.9, func.format
' SOURCE: spec ms-vbal 6.1.2.11.1.9 Format$ identical to Format but typed String
' EXPECT: v=[5]
' EXPECT: t=String
' EXPECT: same=True
Public Sub Main()
    Debug.Print "v=[" & Format$(5) & "]"
    Debug.Print "t=" & TypeName(Format$(5))
    Debug.Print "same=" & CStr(Format$(-2.5) = Format(-2.5))
End Sub
