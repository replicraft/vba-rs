' INV: vbal.5.2.4.2
' SOURCE: spec ms-vbal 5.2.4.2 interface class public members cannot contain an underscore
' ERROR: load
' STRICT_SPEC: true
Public Sub Main()
End Sub
'#MODULE IBad class
Public Sub Do_It()
End Sub
'#MODULE Impl class
Implements IBad
Private Sub IBad_Do_It()
End Sub
