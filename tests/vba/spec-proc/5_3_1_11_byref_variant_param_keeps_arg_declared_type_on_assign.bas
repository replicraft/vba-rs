' INV: vbal.5.3.1.11, decl.byref-byval
' SOURCE: spec ms-vbal 5.3.1.11 ByRef Variant binding is Let-assigned with the referenced variable's declared type
' XFAIL: wrong pending
' ERROR: 13
Sub Clobber(ByRef v As Variant)
    v = "not a number"
End Sub
Public Sub Main()
    Dim n As Long
    Clobber n
End Sub
