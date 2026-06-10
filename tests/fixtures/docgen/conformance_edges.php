<?php
/** API class docs */
class ApiClient {
    /** Public method docs */
    public function fetch($id) {
        return $id;
    }

    private function hidden($id) {
        return $id;
    }
}

/** Public helper docs */
function phpPublicHelper($id) {
    return $id;
}

function phpPublicWithoutDocs($id) {
    return $id;
}
