' INV: vbal.5.6.2.1
' SOURCE: spec ms-vbal 5.6.2.1 Evaluation to a data value, a late-bound member resolving to a subroutine is invoked and its data value is Empty
' EXPECT: ran
' EXPECT: isempty=True
'#MODULE Act class
Public Sub Go()
    Debug.Print "ran"
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim o As Object
    Dim v As Variant
    Set o = New Act
    v = o.Go
    Debug.Print "isempty=" & CStr(IsEmpty(v))
End Sub
