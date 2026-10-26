' INV: vbal.5.3.1.5
' SOURCE: spec ms-vbal 5.3.1.5 ByVal parameter may not carry an array designator
' XFAIL: wrong pending
' ERROR: load
Sub S(ByVal a() As Long)
End Sub
Public Sub Main()
End Sub
