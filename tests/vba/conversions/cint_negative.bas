' INV: func.cint
' SOURCE: derived round half to even also for negatives
' EXPECT: -2
' EXPECT: -2
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(CInt(-1.5))
    Debug.Print CStr(CInt(-2.5))
    Debug.Print CStr(CInt(-0.5))
End Sub
