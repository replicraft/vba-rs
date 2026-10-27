' INV: vbal.5.3.1.3
' SOURCE: derived 5.3.1.3 a function name may carry a type character that gives its result type; callers may omit it
' METHOD: by hand: Twice(4)=8
' EXPECT: 8
' EXPECT: 10
' EXPECT: String
Private Function Twice$(n As Long)
    Twice = CStr(n * 2)
End Function
Public Sub Main()
    Debug.Print Twice$(4)
    Debug.Print Twice(5)
    Debug.Print TypeName(Twice(1))
End Sub
