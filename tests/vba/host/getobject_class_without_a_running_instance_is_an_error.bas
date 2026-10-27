' INV: func.getobject, vbal.6.1.2.8.1.9
' SOURCE: derived the page states an error occurs but not its number; a class-only lookup with no running instance is error 429 (ActiveX component can't create object), unlike the path forms which give 432
' METHOD: by hand; the number 429 is unverified against real VBA and is a candidate for the oracle run
' GRANT: com-create
' EXPECT:  429 
' EXPECT: True
Public Sub Main()
    Dim o As Object
    On Error Resume Next
    Set o = GetObject(, "No.Such.Application")
    Debug.Print Err.Number
    Debug.Print o Is Nothing
End Sub
