' INV: op.imp
' SOURCE: derived a Imp b = (Not a) Or b
' EXPECT: False
' EXPECT: True
' EXPECT: True
' EXPECT: -5
Public Sub Main()
    Debug.Print CStr(True Imp False)
    Debug.Print CStr(False Imp False)
    Debug.Print CStr(False Imp True)
    Debug.Print CStr(5 Imp 3)
End Sub
