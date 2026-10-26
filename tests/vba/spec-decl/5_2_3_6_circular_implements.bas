' INV: vbal.5.2.3.6
' SOURCE: spec ms-vbal 5.2.3.6 circular dependency through Implements directives is illegal
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
End Sub
'#MODULE ClsA class
Implements ClsB
Private Sub ClsB_Hi()
End Sub
'#MODULE ClsB class
Implements ClsA
Private Sub ClsA_Hi()
End Sub
Public Sub Hi()
End Sub
