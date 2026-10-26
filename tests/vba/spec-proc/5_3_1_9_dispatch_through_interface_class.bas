' INV: vbal.5.3.1.9, stmt.implements
' SOURCE: spec ms-vbal 5.3.1.9 call through interface type runs the implementer's method
' EXPECT: impl:ByVal 3
' EXPECT: own
'#MODULE IThing class
Public Sub Work(ByVal n As Long)
End Sub
'#MODULE Impl class
Implements IThing
Private Sub IThing_Work(ByVal x As Long)
    Debug.Print "impl:ByVal " & CStr(x)
End Sub
Public Sub Work(ByVal n As Long)
    Debug.Print "own"
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim i As Impl
    Set i = New Impl
    Dim t As IThing
    Set t = i
    t.Work 3
    i.Work 3
End Sub
