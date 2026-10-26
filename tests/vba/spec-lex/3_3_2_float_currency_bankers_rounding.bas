' INV: vbal.3.3.2, lit.float
' SOURCE: spec ms-vbal 3.3.2 FLOAT tokens, Currency literal fraction rounded to 4 digits with Banker's rounding
' EXPECT: 1.2344
' EXPECT: 1.2346
' EXPECT: 0.0002
' EXPECT: 0.0002
' EXPECT: 0
' EXPECT: 1.5
Public Sub Main()
    Debug.Print CStr(1.23445@)
    Debug.Print CStr(1.23455@)
    Debug.Print CStr(0.00015@)
    Debug.Print CStr(0.00025@)
    Debug.Print CStr(0.00005@)
    Debug.Print CStr(1.5@)
End Sub
