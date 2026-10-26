' INV: vbal.5.6.9.7, op.is
' SOURCE: spec ms-vbal 5.6.9.7 Is operator on Variant operands holding object references
' EXPECT: True
' EXPECT: False
'#MODULE Thing class
Public X As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim a As New Thing, v As Variant, w As Variant
    Set v = a
    Set w = a
    Debug.Print CStr(v Is w)
    Dim b As New Thing
    Debug.Print CStr(v Is b)
End Sub
