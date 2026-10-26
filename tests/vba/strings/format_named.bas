' INV: func.format
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 1,234,567.89
' EXPECT: 12.34%
' EXPECT: 1234.50
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(Format(1234567.891, "Standard"))
    Debug.Print CStr(Format(0.1234, "Percent"))
    Debug.Print CStr(Format(1234.5, "Fixed"))
    Debug.Print CStr(Format(True, "True/False"))
End Sub
