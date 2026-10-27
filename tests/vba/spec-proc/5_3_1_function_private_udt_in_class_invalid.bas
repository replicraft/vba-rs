' INV: vbal.5.3.1
' SOURCE: spec ms-vbal 5.3.1 function in class module may not return a private UDT
' STRICT_SPEC: true
' ERROR: load
'#MODULE Cls class
Private Type Pt
    X As Long
End Type
Public Function F() As Pt
End Function
'#MODULE Module1 standard
Public Sub Main()
End Sub
