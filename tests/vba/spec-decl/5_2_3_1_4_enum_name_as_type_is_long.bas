' INV: vbal.5.2.3.1.4, vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 enum name in an as-type clause means Long
' EXPECT: Long 1000
Private Enum Level
    Low = 1
    High = 2
End Enum
Public Sub Main()
    Dim v As Level
    v = 1000
    Debug.Print TypeName(v) & " " & CStr(v)
End Sub
