' INV: vbal.5.2.1.3
' SOURCE: spec ms-vbal 5.2.1.3 implicit-mode when no Option Explicit
' EXPECT: v=5
' EXPECT: t=Variant/Integer
Public Sub Main()
    undeclared = 5
    Debug.Print "v=" & CStr(undeclared)
    Debug.Print "t=Variant/" & TypeName(undeclared)
End Sub
