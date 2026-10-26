' INV: op.eqv
' SOURCE: derived Eqv = Not (a Xor b)
' EXPECT: True
' EXPECT: False
' EXPECT: -7
Public Sub Main()
    Debug.Print CStr(True Eqv True)
    Debug.Print CStr(True Eqv False)
    Debug.Print CStr(5 Eqv 3)
End Sub
