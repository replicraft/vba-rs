' INV: vbal.5.2.3.1.1
' SOURCE: spec ms-vbal 5.2.3.1.1 typed-variable-dcl element type from suffix
' EXPECT: Long Double String
Dim a&(2)
Dim b#(1 To 2)
Dim c$(3)
Public Sub Main()
    Debug.Print TypeName(a(0)) & " " & TypeName(b(1)) & " " & TypeName(c(0))
End Sub
