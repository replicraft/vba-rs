' INV: op.eqv
' SOURCE: spec ms-vbal 5.6.9.8.5 Eqv operator
' EXPECT: True
' EXPECT: False
' EXPECT: -7
Public Sub Main()
    Debug.Print CStr(True Eqv True)
    Debug.Print CStr(True Eqv False)
    Debug.Print CStr(5 Eqv 3)
End Sub
