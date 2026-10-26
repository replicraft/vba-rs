' INV: vbal.6.1.2.11.1.8, func.format
' SOURCE: spec ms-vbal 6.1.2.11.1.8 Format without a format string, no sign space unlike Str
' EXPECT: f=[5]
' EXPECT: s=[ 5]
' EXPECT: n=[-5]
' EXPECT: d=[1.5]
Public Sub Main()
    Debug.Print "f=[" & Format(5) & "]"
    Debug.Print "s=[" & Str(5) & "]"
    Debug.Print "n=[" & Format(-5) & "]"
    Debug.Print "d=[" & Format(1.5) & "]"
End Sub
